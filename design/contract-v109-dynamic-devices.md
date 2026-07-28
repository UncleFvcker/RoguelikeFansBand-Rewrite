# Contract v109：动态设备身份与首批 staff/wand/rod 激活

## 范围

Contract v109 把原版式通用设备壳的效果身份与容量从 item kind 移到物品实例，并接入首批 wand 伤害、rod 侦测和 staff 治疗。协议为 `1.109`，demo 内容包为 `1.100.0`，state hash Schema 为 `48`，active baseline 包含 373 条 exact fixtures、零 waiver。内置内容 hash 为 `8432e5d6b0143608415de0f49969b6445cd902ef4db58c218c347b5da85cabab`。

## 内容与生成

`ItemDefinition.deviceGeneration.activations` 为一个通用设备 kind 声明 1–256 个候选。每个候选包含稳定 ID、名称键、深度范围、权重、设备检定难度、容量区间、单次成本、目标规格和效果。

- 候选 ID 规范化排序；生成时按实际深度过滤，再使用单一模拟 RNG 做稳定加权选择；
- profile power 固定为 1–100 的生成深度；
- maximum 在候选容量区间内抽取，current 在 `cost..=maximum` 内独立抽取；
- 候选必须共同覆盖 1–100 深度，容量、成本、目标模式和效果必须通过严格内容校验；
- 物化后的 profile、power、难度、成本、目标规格和充能全部保存到 `ItemInstance`，读档不重抽。

demo 新增三种原创设备：

| kind | 首批候选 | 语义 |
| --- | --- | --- |
| Resonance Wand | `spark-bolt`；深度 20 起可加权选中 `frost-bolt` | direction/position/entity，射程 8，复用伤害、抗性、击杀和掉落管线 |
| Resonance Rod | `trap-sense` | self，半径 8，持久揭示带 `trap` tag 的 terrain |
| Resonance Staff | `mending` | self，固定治疗 50，复用 healing resolution |

## 使用事务与知识

动态设备先验证目标模式、范围、line of effect 和目标存在性。非法目标发布 `item.use-unavailable`，不进行设备检定、不抽 RNG、不扣充能，也不推进世界时间。合法目标才进入既有 device skill 检定；失败消耗行动但保留充能，检定成功后按实例 `cost` 扣费并执行效果。

种类未鉴定时，背包 DTO 不公开 activation 或精确充能；`useTargetSpec` 仍始终投影，前端据此进入既有目标选择器，自身目标直接发送。可观察的成功效果使种类 aware，并显示 profile、power、cost 和余量。P58 的静态 `useAction` 保持兼容。

## 协议与存档

`UseItem` 增加可选 `target`；`InventoryItemDto` 增加可选 `activation` 与 `useTargetSpec`。`ItemActivationDto` 保存：

- `profileId` / `nameKey`；
- `power` / `deviceCheckDifficulty` / `cost`；
- 完整 `targetSpec`。

地面、背包、装备和怪物携带四类 save DTO 均保存 activation。载入时必须同时满足当前内容中的 profile 引用、名称键、power 深度范围、难度、成本、目标规格和容量范围；动态 kind 缺少 activation/charges、静态 kind 携带 activation 或任一字段被篡改都会拒绝。旧 content hash 已加入兼容列表；历史物品不会补抽 profile。save 容器保持 v1，新增实例权威状态使 state hash 升至 Schema v48。

## Fixtures

- 369：同一 seed 生成深度 1 和 20 的 wand；浅层固定 `spark-bolt`，深层加权选中 `frost-bolt`，随机容量与回档固定；
- 370：wand 使用 self 错误目标，RNG、world tick 和 12/12 充能均不变；
- 371：wand 命中并击杀实体，成功后充能 12→9，事件、掉落和存档回读固定；
- 372：rod 持久揭示两处 trap terrain，充能 18→13；
- 373：staff 从 1 HP 治疗至上限，充能 24→14。

368 条 v108 场景迁移到 Schema v48 后保持既有命令语义。contract 测试提供 `generationDepth` 调试前置条件，由核心真实生成器物化设备，用于固定深度、权重和容量，不复制生成算法。

## Legacy 导入结果

真实导入为原版通用 wand/staff/rod 壳生成首批动态候选；scroll 仍保留行为缺口。原版 `f_info` 的 `TRAP` 旗标同时映射为 terrain `trap` tag，使 rod 侦测候选通过严格引用校验。重新生成的本地包包含 937 items、128 affixes、1260 abilities、4 ability books，`device-effect` 由 64 降至 61，content hash 为 `68f8c65c4b80e67437457e1c51ff77b11c2d4a095bb2e9cfa01983c244d427b3`。

## 后续候选

P60 应先对照原版区分 staff/wand/rod 的恢复、rod 时间和失败/强行使用语义，再建立通用 recharge 事务。之后按真实缺口收益扩展 `device-effect` 61、`artifact-activation` 180、`ego-activation` 13 或剩余 `consumable-effect` 89；目标取消、desperation 和再充能破坏风险必须分别形成明确契约。
