# Contract v130：Genocide 卷轴

日期：2026-07-29

Contract v130 接入原版卷轴 sval 44 的 Genocide。协议升级为 `1.120`，demo 内容包为 `1.121.0`，save 容器保持 v1，state hash Schema 保持 `53`。active baseline 包含 433 条 exact fixtures、零 waiver，内置内容 hash 为 `786aba7f693bac066d6caa0dbc848c97ac7bc01e4652bfeb2674cfa739130549`。

## 1. 效果与输入边界

内容层新增 `genocide { power }` 物品效果，demo 与 legacy sval 44 均使用 power 300。使用者必须提供恰好一个非控制 Unicode scalar 作为 glyph；缺失、多个字符或控制字符在消费、时间和 RNG 前返回普通 `item.use-unavailable`。planner 只保存 glyph，不提前冻结 actor ID。

合法使用在执行阶段收集当前楼层全部存活、actor definition glyph 与输入完全相同的实体，再按稳定实体 ID 顺序复用既有 Genocide 结算：

1. 每个候选先抽 `1d4` 玩家疲劳；
2. `unique` 或 `guardian` 必定抵抗，且不再抽 power 对抗；
3. 其他目标抽 `bounded(power)`，actor level 大于结果时抵抗，否则直接移除；
4. 移除不走普通击杀事务，因此不产生 XP、掉落、尸体、任务进度或守护者胜利。

合法 glyph 没有候选时仍消费、推进时间并写入 Tried + Aware，但不抽效果 RNG。实际移除、全部抵抗和空结果均使用同一结构化 `item.use-genocide` 事件。

## 2. 协议、存档与界面

协议新增窄命令 `UseItemByGlyph { itemId, glyph }`，核心入口立即将其归一为既有物品使用动作；没有扩展通用 `TargetMode`。`InventoryItemDto` 增加省略式 `requiresTargetGlyph`，Web 只在该字段为 true 时打开单字符输入对话框并发送新命令。

glyph 选择是单次命令输入，不进入存档或 state hash；Genocide 复用既有 actor 状态与移除结果，因此 state hash Schema 不升级。Web 只增加对话框、事件格式化和中英文 Fluent 文案，没有新增专门 E2E 场景。

## 3. 导入与契约

legacy importer 将 tval 70 / sval 44 映射为 `genocide { power: 300 }`。固定原版 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 严格导入成功，`scroll-effect` 从 18 降至 17；真实包保持 937 items、122 affixes、1260 abilities、4 ability books、1332 actors 和 68 races，源码校验、编译与二进制回读 hash 均为 `4814e2cd4a0d8ac582c1b514e1cbc7998760cbe26f6293a6ab5bd5ff5324707a`。

fixture 433 固定 glyph `o` 的单目标移除、`1d4` 疲劳、消费、知识、事件和存档回读。一个窄核心单测覆盖缺失/非法 glyph 的零时间零 RNG 拒绝，以及合法空选择的消费、Aware 和零效果 RNG；导入器既有表测试增加 sval 44。

既有 432 条 fixture 只更新 `stateHash` 与 `saveRoundTripStateHash`。

## 4. 明确遗留

- 原版 `_scroll_power`、Devicemaster Scrolls、任务/竞技场禁用、`NOGENO`、questor、骑乘和 virtues 尚未接入；
- 原版输入阶段的 `?` 帮助页及 `n`/`X`/非字母确认提示未复刻；当前 Web 将合法单字符按字面 glyph 提交；
- 当前 glyph 来自 actor definition，不处理运行时变形改变显示 glyph 的情形；
- Genocide 只复用现有稳定候选结算，不新增通用 glyph target、actor selector 或 actor-removal 框架；
- 非卷轴来源和批量/历史 glyph 选择不在本轮；
- 剩余 17 个 `scroll-effect` 继续按世界/地形、状态和物品/成长事务拆分。
