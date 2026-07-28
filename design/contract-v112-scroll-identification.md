# Contract v112：卷轴效果重分类与首批鉴定事务

状态：已实现

Contract v112 把旧版导入报告中误归为设备的卷轴效果独立为 `scroll-effect`，并用普通鉴定与完全鉴定完成首批卷轴事务纵切。协议为 `1.112`，demo 内容包为 `1.103.0`，state hash Schema 保持 `49`，active baseline 包含 386 条 exact fixtures、零 waiver。内置内容 hash 为 `c02d577a3eaf36f61c636c1b8bbdfcfa30935aef08ec4d9c5b59e77ef21b4d25`。

## 1. 内容模型

`ItemUseEffectDefinition` 新增 `identify-item { full }`：

- `full: false` 执行普通鉴定，把目标实例推进到 `appraised`；
- `full: true` 执行完全鉴定，把目标实例推进到 `identified`，并揭示固定和动态 materialized affix；
- 固定 `useAction` 与动态 `deviceGeneration.activations` 都可以声明该效果，但目标规格必须且只能是 `item`；
- 恢复型 `sequence` 不接受鉴定效果，避免把物品目标事务嵌入自目标恢复序列。

内容编译器继续验证 `useAction`、动态 activation、目标规格、充能和堆叠边界。demo 新增 Appraisal Scroll 与 Revelation Scroll，两者可堆叠、共享未知卷轴外观，成功使用后分别承载普通和完整鉴定。

## 2. 事务与知识语义

- 来源卷轴必须是玩家背包中数量大于零的实例；目标必须是另一件当前可访问的物品实例。核心接受背包、装备和玩家脚下的地面物品；Web 首版选择器列出背包与已装备物品。
- 缺少目标、目标不存在、目标不可访问或把来源卷轴自身作为目标，均在物品消耗、正式 RNG 和 world tick 前拒绝，返回 `item.use-unavailable`。
- 成功后只消耗来源堆叠的一件；剩余堆叠保留同一实例 ID。鉴定本身不抽 RNG。
- 普通鉴定写入目标实例的 `appraised`，允许显示品质，但不揭示 affix。
- 完全鉴定同时写入 `appraised`、`identified` 和完整 affix 知识；重复鉴定允许成功消费，但 `changed=false`。
- 成功使用使来源卷轴种类变为 `aware`。目标种类与实例知识复用既有 `itemKnowledge` / `itemPropertyKnowledge`，不建立楼层或携带状态依赖。
- Death Esoteria 的鉴定效果改为复用同一 `identify_item_instance` helper，维持法术自己的 full roll 和事件外壳。

结构化事件新增 `GameEventOutcomeDto::ItemIdentify { resolution }`。`ItemIdentifyResolutionDto` 固定 `itemId`、`itemKindId`、`full` 和 `changed`；事件 kind 为 `item.use-identified` 或 `item.use-fully-identified`。

## 3. Web 交互

Web 增加通用物品目标对话框，按稳定实例 ID 列出背包和装备。鉴定卷轴与 Death 鉴定能力共用该对话框；确认后发送既有 `TargetSelection.item`，取消不发送命令。中英文名称、说明、按钮和事件文本均由 Fluent 提供。

## 4. Legacy 导入

固定来源 commit `191f48c3fd1cdbc81a3d3395a88cd6758402b4d9` 的 tval 70/71 缺口统一重命名为 `scroll-effect`，不再把卷轴计作 `device-effect`。首批按原版 sval 映射：

- sval 12：普通鉴定；
- sval 13：完全鉴定。

独立 detached worktree 上的真实导入结果为 937 items、128 affixes、1260 abilities 和 4 ability books；`scroll-effect` 从 61 降至 59，`device-effect` 不再出现在报告。严格源校验、编译和编译产物回读的 content hash 均为 `143ed91ebd453dd22628548663dac0483c28d2f20625b749844a5419c61cac44`。

## 5. 存档、回放与 fixtures

鉴定继续写入 save v1 既有 `itemKnowledge` 与 `itemPropertyKnowledge`；来源数量使用既有四类物品实例保存。协议新增的是事件 DTO，不新增存档字段，因此 state hash Schema 保持 49。旧 built-in 内容 hash 加入迁移白名单，旧档不补发卷轴、不补鉴定、不抽 RNG。

fixtures 384–386 固定：

- 普通鉴定消费一张卷轴、来源变 aware、目标变 appraised 且零鉴定 RNG；
- 完全鉴定保留剩余堆叠、目标变 identified，并完成 save round-trip；
- 不存在目标和自身目标连续拒绝，卷轴、知识、RNG 与 world tick 均不变。

## 6. 后续边界

剩余 59 条 `scroll-effect` 应继续按覆盖收益拆成传送/回城、侦测/地图、附魔/强化、诅咒与召唤等事务族。批量鉴定、自动选择、脚下地面物品的 Web 选择入口、商店鉴定服务和鉴定失败率均不在首版范围。`artifact-activation` 180、`ego-activation` 13 与 `consumable-effect` 81 继续保留，新增主动效果应复用既有实例事务和知识边界。
